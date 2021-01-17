#![no_std]
#![no_main]

use cortex_m_semihosting::{debug, hprintln};
// use panic_halt as _;
use panic_semihosting as _;
use rtic::{app,  cyccnt::{Instant, U32Ext as _}};

use stm32f3xx_hal::{
    prelude::*,
    // stm32::{self, USART1},
    serial::{Serial, Rx, Tx},
    // time::{MonoTimer, MilliSeconds},
    gpio,
};
use stm32f3xx_hal::stm32::Interrupt;
use nb::block;

use switch_hal::{Switch, ActiveHigh, OutputSwitch, IntoSwitch};

use rosserial_rs::msgs::*;
use rosserial_rs::ros;

const RX_SZ: usize = 512;

enum State {
    Sync,
    ProtocolVersion,
    SizeLsb,
    SizeMsb,
    SizeChecksum,
    TopicIdLsb,
    TopicIdMsb,
    Message,
    MessageChecksum
}

const PROTOCOL_VER1: u8 = 0xFF;
const PROTOCOL_VER2: u8 = 0xFE;


// Serial buffer
pub struct Buffer {
    buffer: [u8; RX_SZ],
    write_idx: usize,
    read_idx: usize,
}

impl Buffer {
    const fn new() -> Buffer {
        Buffer {
            buffer: [0; RX_SZ],
            write_idx: 0,
            read_idx: 0,
        }
    }

    pub fn push(&mut self, data: u8) {
        self.buffer[self.write_idx] = data;
        self.write_idx = (self.write_idx + 1) % RX_SZ;
    }

    pub fn read(&mut self) -> Option<u8> {
        if self.write_idx != self.read_idx {
            let data = self.buffer[self.read_idx];
            self.read_idx = (self.read_idx + 1) % RX_SZ;

            Some(data)
        }
        else {
            None
        }
    }
}

#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        #[init(Buffer::new())]
        rxbuf: Buffer,
        tx: Tx<stm32f3xx_hal::pac::USART1>,
        rx: Rx<stm32f3xx_hal::pac::USART1>,
        led1: Switch<gpio::gpioe::PEx<gpio::Output<stm32f3xx_hal::gpio::PushPull>>, switch_hal::ActiveHigh>,
        led2: Switch<gpio::gpioe::PEx<gpio::Output<stm32f3xx_hal::gpio::PushPull>>, switch_hal::ActiveHigh>
    }

    #[init(spawn = [spin])]
    fn init(mut cx: init::Context) -> init::LateResources {
        let device = cx.device;
        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        // Setup USART1
        let mut gpioc = device.GPIOC.split(&mut rcc.ahb);

        let tx = gpioc.pc4.into_af7(&mut gpioc.moder, &mut gpioc.afrl);
        let rx = gpioc.pc5.into_af7(&mut gpioc.moder, &mut gpioc.afrl);

        let mut serial = Serial::usart1(device.USART1, (tx, rx), 115_200.bps(), clocks, &mut rcc.apb2);

        // Enable RX interrupt
        serial.listen(stm32f3xx_hal::serial::Event::Rxne);

        let (tx, rx) = serial.split();

        // debug led
        let mut gpioe = device.GPIOE.split(&mut rcc.ahb);
        let mut led1 = gpioe.pe9
            .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper)
            .downgrade()
            .into_active_high_switch();
        let mut led2 = gpioe.pe13
            .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper)
            .downgrade()
            .into_active_high_switch();

        led1.off().ok();
        led2.off().ok();

        // Enable timer
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Spawn
        cx.spawn.spin().unwrap();

        // pend all used interrupts
        rtic::pend(Interrupt::USART1_EXTI25);

        init::LateResources {
            tx,
            rx,
            led1,
            led2,
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(resources = [rxbuf, tx, led1, led2])]
    fn spin(mut cx: spin::Context) {
        let mut state = State::Sync;
        let mut message_out = [0x00u8; 1024];
        let mut message_in = [0x00u8; 1024];
        let mut index = 0;
        let mut bytes: u16 = 0;
        let mut topic: u16 = 0;
        let mut checksum: u16 = 0;
        let mut configured: bool = false;

        let mut publishers: [Option<ros::Publisher>; 256] = [None; 256];

        let mut last_sync = Instant::now();
        let mut last_pub = Instant::now();

        // advertise...
        publishers[0] = Some(advertise::<std_msgs::Bool>("test", 100));

        loop {
            let current_time = Instant::now();

            let data = cx.resources.rxbuf.lock(|buf|{
                buf.read()
            });

            if let Some(data) = data {
                match state {
                    State::Sync => {
                        if data == 0xFF {
                            state = State::ProtocolVersion;
                        }
                    },
                    State::ProtocolVersion => {
                        state = if data == PROTOCOL_VER2 {
                            State::SizeLsb
                        }
                        else {
                            State::Sync
                        };
                    },
                    State::SizeLsb => {
                        bytes = data as u16;
                        index = 0;
                        checksum = data as u16; // first byte to calculate checksum
                        state = State::SizeMsb;
                    },
                    State::SizeMsb => {
                        bytes |= (data as u16) << 8;
                        state = State::SizeChecksum;
                    },
                    State::SizeChecksum => {
                        // Message Length Checksum = 255 - ((Message Length High Byte + Message Length Low Byte) % 256 )
                        // TODO(nnarain): This doesn't work?
                        // state = if checksum % 256 == 255 {
                        //     cx.resources.led1.on().ok();
                        //     State::TopicIdLsb
                        // }
                        // else {
                        //     cx.resources.led2.on().ok();
                        //     State::Sync
                        // };
                        state = State::TopicIdLsb;
                    },
                    State::TopicIdLsb => {
                        topic = data as u16;
                        checksum = data as u16;
                        state = State::TopicIdMsb;
                    },
                    State::TopicIdMsb => {
                        topic |= (data as u16) << 8;
                        state = if bytes == 0 { State::MessageChecksum } else { State::Message };
                    },
                    State::Message => {
                        message_in[index] = data;
                        index += 1;
                        bytes -= 1;

                        if bytes == 0 {
                            state = State::MessageChecksum;
                        }
                    },
                    State::MessageChecksum => {
                        // TODO: checksum

                        if topic == rosserial_msgs::TOPICINFO_ID_PUBLISHER {
                            request_sync_time(cx.resources.tx);
                            negotiate_topics(&publishers, cx.resources.tx);

                            configured = true;
                        }
                        else if topic == rosserial_msgs::TOPICINFO_ID_TIME {
                            // TODO: sync time
                            cx.resources.led1.on().ok();
                        }
                        else if topic == rosserial_msgs::TOPICINFO_ID_TX_STOP {
                            configured = false;
                        }

                        state = State::Sync;
                    },
                }
            }

            if current_time > last_sync + 5_000_000.cycles() {
                request_sync_time(cx.resources.tx);
                last_sync = current_time;
            }

            if current_time > last_pub + 5_000_000.cycles() {
                // pub
                let mut msg = std_msgs::Bool::default();
                msg.data = true;

                if let Some(ref publisher) = publishers[0] {
                    cx.resources.led2.on().ok();
                    publish(publisher.id, &msg, cx.resources.tx);
                    last_pub = current_time;
                }
            }
        }
    }



    // Interrupt handler for serial receive, needs to be high priority or the receive buffer overruns
    #[task(binds=USART1_EXTI25, priority = 2, resources=[rx, rxbuf])]
    fn USART1(cx: USART1::Context) {
        if let Ok(c) = cx.resources.rx.read() {
            cx.resources.rxbuf.push(c);
        }
    }

    // spare interrupt used for scheduling software tasks
    extern "C" {
        fn USART2_EXTI26();
    }
};

fn request_sync_time(tx: &mut Tx<stm32f3xx_hal::pac::USART1>) {
    let time = std_msgs::Time::default();
    publish(rosserial_msgs::TOPICINFO_ID_TIME, &time, tx)
}

fn advertise<Msg: Message>(topic: &'static str, id: u16) -> ros::Publisher {
    ros::Publisher::new(topic, id, Msg::name(), Msg::md5())
}

fn negotiate_topics(pubs: &[Option<ros::Publisher>; 256], tx: &mut Tx<stm32f3xx_hal::pac::USART1>) {
    let mut ti = rosserial_msgs::TopicInfo::default();
    for p in pubs {
        if let Some(ref p) = p {
            ti.id = p.id;
            ti.name = p.topic;
            ti.message_type = p.message_type;
            ti.md5 = p.md5sum;
            ti.buffer_size = 256;

            publish(rosserial_msgs::TOPICINFO_ID_PUBLISHER, &ti, tx);
        }
    }
}

fn publish(topic_id: u16, msg: &dyn Message, tx: &mut Tx<stm32f3xx_hal::pac::USART1>) {
    let mut message_out: [u8; 256] = [0; 256];

    let len = msg.serialize(&mut message_out[7..]);

    message_out[0] = 0xFF;
    message_out[1] = PROTOCOL_VER2;
    message_out[2] = (len & 0xFF) as u8;
    message_out[3] = ((len >> 8) & 0xFF) as u8;
    message_out[4] = 255 - ((message_out[2] as u16 + message_out[3] as u16) % 256) as u8;
    message_out[5] = (topic_id & 0xFF) as u8;
    message_out[6] = ((topic_id >> 8) & 0xFF) as u8;

    // TODO: Want to use iterator sum here..
    let last = 7 + len as usize;
    let mut checksum: u32 = 0;
    for b in &message_out[5..last] {
        checksum += *b as u32;
    }
    message_out[last] = (255 - (checksum as u16 % 256)) as u8;

    for b in &message_out[..last+1] {
        block!(tx.write(*b)).ok();
    }
}
