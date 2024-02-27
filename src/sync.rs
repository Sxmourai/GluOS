pub trait TimeOutMutex<'a, T> {
    type Guard;
    fn try_lock_with_timeout(&'a self) -> Option<Self::Guard>;
    fn lock_with_timeout(&'a self) -> Self::Guard {
        self.try_lock_with_timeout().expect("Locking timed out !")
    }
}

impl<'a, M: 'a> TimeOutMutex<'a, M> for spin::Mutex<M> {
    type Guard = spin::MutexGuard<'a, M>;
    fn try_lock_with_timeout(&'a self) -> Option<Self::Guard> {
        for i in 0..100_000 {
            if !self.is_locked() {
                return Some(self.lock())
            }
            core::hint::spin_loop()
        }
        None
    }
}

pub trait TimeOutRwLock<'a, T> {
    type ReadGuard;
    type WriteGuard;
    fn try_read_with_timeout(&'a self) -> Option<Self::ReadGuard>;
    fn try_write_with_timeout(&'a self) -> Option<Self::WriteGuard>;
    fn read_with_timeout(&'a self) -> Self::ReadGuard {
        self.try_read_with_timeout().expect("Reading timed out !")
    }
    fn write_with_timeout(&'a self) -> Self::WriteGuard {
        self.try_write_with_timeout().expect("Writing timed out !")
    }
}

impl<'a, M: 'a> TimeOutRwLock<'a, M> for spin::RwLock<M> {
    type ReadGuard = spin::RwLockReadGuard<'a, M>;
    fn try_read_with_timeout(&'a self) -> Option<Self::ReadGuard> {
        timeout(&|| self.writer_count()==0, &||self.read())
    }
    type WriteGuard = spin::RwLockWriteGuard<'a, M>;
    fn try_write_with_timeout(&'a self) -> Option<Self::WriteGuard> {
        timeout(&|| self.writer_count()==0&&self.reader_count()==0, &||self.write())
    }
}
fn timeout<T>(check_avail: &dyn Fn() -> bool, on_avail: &dyn Fn() -> T) -> Option<T> {
    for i in 0..100_000 {
        if (check_avail)() {
            return Some((on_avail)())
        }
        core::hint::spin_loop()
    }
    None
}