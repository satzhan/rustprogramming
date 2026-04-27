struct MyDeque<T> {
    buf: Vec<Option<T>>,  // backing storage; None = empty slot
    head: usize,          // index of the logical front
    len: usize,           // how many slots are occupied
    cap: usize,           // total slots (== buf.len())
}

impl<T: std::fmt::Debug> MyDeque<T> {
    fn new() -> Self {
        let cap = 4;
        let mut buf = Vec::with_capacity(cap);
        for _ in 0..cap { buf.push(None); }
        MyDeque { buf, head: 0, len: 0, cap }
    }

    fn push_back(&mut self, item: T) {
        if self.len == self.cap { self.grow(); }
        let slot = (self.head + self.len) % self.cap;  // next free slot
        self.buf[slot] = Some(item);
        self.len += 1;
    }
    fn push_front(&mut self, item: T) {
        if self.len == self.cap { self.grow(); }
        self.head = (self.head + self.cap - 1) % self.cap;
        self.buf[self.head] = Some(item);
        self.len += 1;
    }
    fn pop_front(&mut self) -> Option<T> {
        if self.len == 0 { return None; }
        let item = self.buf[self.head].take();          // leaves None behind
        self.head = (self.head + 1) % self.cap;         // advance head, wrap
        self.len -= 1;
        item
    }

    fn grow(&mut self) {
        let new_cap = self.cap * 2;
        println!("  [grow] cap {} -> {}, head was {}", self.cap, new_cap, self.head);
        let mut new_buf: Vec<Option<T>> = Vec::with_capacity(new_cap);
        for _ in 0..new_cap { new_buf.push(None); }
        for i in 0..self.len {
            let old = (self.head + i) % self.cap;
            new_buf[i] = self.buf[old].take();
        }
        self.buf = new_buf;
        self.cap = new_cap;
        self.head = 0;                                  // unwound; head resets
    }

    fn show(&self, label: &str) {
        let physical: Vec<String> = self.buf.iter()
            .map(|s| s.as_ref().map_or("_".to_string(), |v| format!("{:?}", v)))
            .collect();
        let mut logical = Vec::new();
        for i in 0..self.len {
            let idx = (self.head + i) % self.cap;
            if let Some(v) = &self.buf[idx] { logical.push(format!("{:?}", v)); }
        }
        println!("{:<22} physical=[{}]  logical=[{}]  head={} len={} cap={}",
            label, physical.join(","), logical.join(","), self.head, self.len, self.cap);
    }
}

fn main() { // deque unwrap
    let mut d: MyDeque<i32> = MyDeque::new();
    d.show("start");

    for i in 1..=4 { d.push_back(i); }
    d.show("after push 1..4");

    d.pop_front();
    d.pop_front();
    d.show("after 2 pops");        // head moved right, two slots now empty at front

    d.push_back(5);
    d.push_back(6);
    d.show("after push 5,6");      // wraps: 5,6 land at indices 0,1

    d.push_back(7);                // full -> triggers grow + unwind
    d.show("after push 7");

    for i in 8..=12 { d.push_front(i); }
    d.show("after push 8..12");    // another grow somewhere in here

    while let Some(_) = d.pop_front() {}
    d.show("after drain");
}