#![no_std]
#![no_main]

// use cortex_m_semihosting::{debug, hprintln};
// use panic_halt as _;
use panic_semihosting as _;

use stm32f3xx_hal::stm32::Interrupt;

use rtic::{app,  cyccnt::{Instant, U32Ext as _}};

use stm32f3discovery::hardware::{Hardware, SerialRx, SerialTx, Led};

use rosserial::msgs::*;
use rosserial::ros::{HardwareInterface, NodeHandle, Subscriber};

use switch_hal::OutputSwitch;

use heapless::spsc::Queue;
use heapless::consts::*;

struct SpinInstance<'a> {
    // rx: &'a SerialRx,
    tx: &'a mut SerialTx,
    data: Option<u8>,
}

impl<'a> SpinInstance<'a> {
    pub fn new(tx: &'a mut SerialTx, data: Option<u8>) -> Self {
        SpinInstance {
            tx,
            data,
        }
    }
}

impl HardwareInterface for SpinInstance<'_> {
    fn read(&mut self) -> Option<u8> {
        // self.rx.read()
        self.data
    }

    fn write(&mut self, data: u8) {
        self.tx.write(data);
    }

    fn time(&self) -> u32 {
        0
    }
}

#[app(device = stm32f3xx_hal::pac, peripherals = true, monotonic = rtic::cyccnt::CYCCNT)]
const APP: () = {
    struct Resources {
        rx: SerialRx,
        tx: SerialTx,
        led: Led,
    }

    #[init(spawn = [spin])]
    fn init(mut cx: init::Context) -> init::LateResources {
        // Enable timer
        cx.core.DCB.enable_trace();
        cx.core.DWT.enable_cycle_counter();

        // Spawn
        cx.spawn.spin().unwrap();

        // pend all used interrupts
        rtic::pend(Interrupt::USART1_EXTI25);

        let (rx, tx, led) = Hardware::initialize(cx.device).split();

        init::LateResources {
            rx,
            tx,
            led,
        }
    }

    #[idle]
    fn idle(_: idle::Context) -> ! {
        loop {}
    }

    #[task(resources = [rx, tx, led])]
    fn spin(mut cx: spin::Context) {
        let mut last_sync = Instant::now();
        let mut last_pub = Instant::now();

        let mut led_cmd_queue: Queue<bool, U235, _> = Queue::u8();
        let (mut cmd_in, mut cmd_out) = led_cmd_queue.split();

        let mut bool_sub = Subscriber::new("led_cmd", move |msg: std_msgs::Bool| {
            cmd_in.enqueue(msg.data).unwrap();
        });

        let mut nodehandle = NodeHandle::default();
        let test_pub = nodehandle.advertise::<std_msgs::Bool>("test").unwrap();
        nodehandle.register_subscriber::<_, std_msgs::Bool>(&mut bool_sub);

        loop {
            let current_time = Instant::now();

            let rx_data = cx.resources.rx.lock(|rx| {
                rx.read()
            });

            let mut spin_data = SpinInstance::new(&mut cx.resources.tx, rx_data);
            nodehandle.spin_once(&mut spin_data);

            if current_time > last_sync + 5_000_000.cycles() {
                nodehandle.request_sync_time(&mut spin_data);
                last_sync = current_time;
            }

            if current_time > last_pub + 5_000_000.cycles() {
                let mut msg = std_msgs::Bool::default();
                msg.data = true;

                nodehandle.publish(test_pub, &msg, &mut spin_data);
                last_pub = current_time;
            }

            // led cmd events
            if let Some(cmd) = cmd_out.dequeue() {
                if cmd {
                    cx.resources.led.on().ok();
                }
                else {
                    cx.resources.led.off().ok();
                }
            }
        }
    }

    // Interrupt handler for serial receive, needs to be high priority or the receive buffer overruns
    #[task(binds=USART1_EXTI25, priority = 2, resources=[rx])]
    fn USART1(cx: USART1::Context) {
        cx.resources.rx.update();
    }

    // spare interrupt used for scheduling software tasks
    extern "C" {
        fn USART2_EXTI26();
        fn EXTI0();
    }
};
