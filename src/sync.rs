pub trait TimeOutMutex<'a, T> {
    type Guard;
    #[track_caller]
    fn try_lock_with_timeout(&'a self) -> Option<Self::Guard>;
    #[track_caller]
    fn lock_with_timeout(&'a self) -> Self::Guard {
        return self.try_lock_with_timeout().expect("Locking timed out !")
    }
}

impl<'a, M: 'a> TimeOutMutex<'a, M> for spin::Mutex<M> {
    type Guard = spin::MutexGuard<'a, M>;
    #[track_caller]
    fn try_lock_with_timeout(&'a self) -> Option<Self::Guard> {
        timeout(&|| return !self.is_locked(), &|| return self.lock())
    }
}

pub trait TimeOutRwLock<'a, T> {
    type ReadGuard;
    type WriteGuard;
    #[track_caller]
    fn try_read_with_timeout(&'a self) -> Option<Self::ReadGuard>;
    #[track_caller]
    fn try_write_with_timeout(&'a self) -> Option<Self::WriteGuard>;
    #[track_caller]
    fn read_with_timeout(&'a self) -> Self::ReadGuard {
        return self.try_read_with_timeout().expect("Reading timed out !")
    }
    #[track_caller]
    fn write_with_timeout(&'a self) -> Self::WriteGuard {
        return self.try_write_with_timeout().expect("Writing timed out !")
    }
}

impl<'a, M: 'a> TimeOutRwLock<'a, M> for spin::RwLock<M> {
    type ReadGuard = spin::RwLockReadGuard<'a, M>;
    #[track_caller]
    fn try_read_with_timeout(&'a self) -> Option<Self::ReadGuard> {
        timeout(&|| return self.writer_count()==0, &||return self.read())
    }
    type WriteGuard = spin::RwLockWriteGuard<'a, M>;
    #[track_caller]
    fn try_write_with_timeout(&'a self) -> Option<Self::WriteGuard> {
        timeout(&|| return self.writer_count()==0&&self.reader_count()==0, &||return self.write())
    }
}
#[track_caller]
fn timeout<T>(check_avail: &dyn Fn() -> bool, on_avail: &dyn Fn() -> T) -> Option<T> {
    for i in 0..100_000 {
        if (check_avail)() {
            return Some((on_avail)())
        }
        core::hint::spin_loop()
    }
    log::error!("Mutex/RwLock timeout at {}:{}:{}", file!(), line!(), column!());
    return None
}