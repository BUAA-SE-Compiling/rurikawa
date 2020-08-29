use std::cell::RefCell;

pub struct FlowSnake(pub u64);

thread_local! {
static last_generation_time: RefCell<u64> = RefCell::new(0);
static seq_number: RefCell<u64> = RefCell::new(0);
static loc_worker_id: RefCell<once_cell::unsync::Lazy<u64>> =
    RefCell::new(once_cell::unsync::Lazy::new(get_worker_id));
}

fn get_worker_id() -> u64 {
    rand::random::<u64>()
}

pub const timestamp_bits: u32 = 34;
pub const worker_id_bits: u32 = 12;
pub const sequence_bits: u32 = 18;

impl FlowSnake {
    pub fn new_parts(timestamp: u64, worker_id: u64, seq: u64) -> FlowSnake {
        let n = ((timestamp & ((1 << timestamp_bits) - 1)) << (worker_id_bits + sequence_bits))
            | ((worker_id & ((1 << worker_id_bits) - 1)) << (sequence_bits))
            | (seq & ((1 << sequence_bits) - 1));
        FlowSnake(n)
    }

    pub fn generate() -> FlowSnake {
        let time = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let worker_id = loc_worker_id.with(|x| **x.borrow_mut());
        let seq = if last_generation_time.with(|x| time <= *x.borrow()) {
            seq_number.with(|s| {
                let mut s = s.borrow_mut();
                let seq = *s;
                *s += 1;
                seq
            })
        } else {
            last_generation_time.with(|t| *(t.borrow_mut()) = time);
            let rnd = rand::random::<u64>() % ((1 << sequence_bits) - (1 << (sequence_bits - 2)));
            seq_number.with(|s| {
                (*s.borrow_mut()) = rnd + 1;
            });
            rnd
        };

        FlowSnake::new_parts(time, worker_id, seq)
    }
}
