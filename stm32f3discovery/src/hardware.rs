use stm32f3xx_hal::{
    prelude::*,
    serial::{Serial, Rx, Tx},
    pac::Peripherals,
    gpio,
};
use nb::block;

use switch_hal::{Switch, OutputSwitch, IntoSwitch};

const RX_SZ: usize = 512;


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


pub type Led = Switch<gpio::gpioe::PEx<gpio::Output<stm32f3xx_hal::gpio::PushPull>>, switch_hal::ActiveHigh>;

pub struct SerialRx {
    rx: Rx<stm32f3xx_hal::pac::USART1>,
    buf: Buffer,
}

impl SerialRx {
    pub fn new(rx: Rx<stm32f3xx_hal::pac::USART1>) -> Self {
        SerialRx {
            rx,
            buf: Buffer::new(),
        }
    }

    pub fn update(&mut self) {
        if let Ok(data) = self.rx.read() {
            self.buf.push(data);
        }
    }

    pub fn read(&mut self) -> Option<u8> {
        self.buf.read()
    }
}

pub struct SerialTx {
    tx: Tx<stm32f3xx_hal::pac::USART1>,
}

impl SerialTx {
    pub fn new(tx: Tx<stm32f3xx_hal::pac::USART1>) -> Self {
        SerialTx {
            tx,
        }
    }

    pub fn write(&mut self, data: u8) {
        block!(self.tx.write(data)).ok();
    }
}

pub struct Hardware {
    rx: SerialRx,
    tx: SerialTx,
    led1: Led,      // Debug LED 1
    // led2: Led,      // Debug LED 2
}

impl Hardware {
    pub fn initialize(device: Peripherals) -> Self {
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
        // let mut led2 = gpioe.pe13
        //     .into_push_pull_output(&mut gpioe.moder, &mut gpioe.otyper)
        //     .downgrade()
        //     .into_active_high_switch();

        led1.off().ok();
        // led2.off().ok();

        Hardware {
            rx: SerialRx::new(rx),
            tx: SerialTx::new(tx),
            led1,
            // led2,
        }
    }

    pub fn split(self) -> (SerialRx, SerialTx, Led) {
        (self.rx, self.tx, self.led1)
    }
}
