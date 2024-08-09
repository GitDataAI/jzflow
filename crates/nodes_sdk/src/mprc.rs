use rand::Rng;
use std::{
    borrow::Borrow,
    hash::Hash,
    vec,
};

pub struct Mprs<K, T>
where
    T: Clone,
{
    entries: Vec<(K, T)>,
}

impl<K, T> Default for Mprs<K, T>
where
    T: Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, T> Mprs<K, T>
where
    T: Clone,
{
    pub fn new() -> Self {
        Mprs { entries: vec![] }
    }

    pub fn remove<Q>(&mut self, k: &Q) -> Option<T>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        for i in 0..self.entries.len() {
            if self.entries[i].0.borrow() == k {
                return Some(self.entries.swap_remove(i).1);
            }
        }

        None
    }

    pub fn count(&self) -> usize {
        self.entries.len()
    }

    pub fn iter(&mut self) -> impl Iterator<Item = &(K, T)> {
        self.entries.iter()
    }

    pub fn insert(&mut self, k: K, sender: T) -> Option<T>
    where
        K: Hash + Eq,
    {
        let ret = self.remove(&k);
        self.entries.push((k, sender));

        ret
    }

    pub fn get_random(&mut self) -> Option<&(K, T)>
    where
        K: Hash + Eq,
    {
        let count = self.entries.len();
        if count == 0 {
            return None;
        }
        let mut rng = rand::thread_rng();
        let rand_one = rng.gen_range(0..count);
        Some(&self.entries[rand_one])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{
                AtomicUsize,
                Ordering,
            },
            Arc,
        },
        time::Duration,
    };
    use tokio::{
        sync::mpsc,
        time::sleep,
    };

    #[tokio::test]
    async fn test_mprs_insert_remove() {
        let mut mprs = Mprs::<i32, mpsc::Sender<u32>>::new();
        for i in 0..5 {
            let (tx, _) = mpsc::channel(10);
            mprs.insert(i, tx);
        }

        let keys: Vec<i32> = mprs.iter().map(|a| a.0).collect();
        assert_eq!(vec![0, 1, 2, 3, 4], keys);

        let _ = mprs.remove(&3).unwrap();
        let keys: Vec<i32> = mprs.iter().map(|a| a.0).collect();
        assert_eq!(vec![0, 1, 2, 4], keys);
    }
    #[tokio::test]
    async fn test_mprs_send_value() {
        let mut mprs = Mprs::<String, mpsc::Sender<u32>>::new();

        let counter = Arc::new(AtomicUsize::new(0));
        {
            let (tx, mut rx) = mpsc::channel(10);
            let counter = counter.clone();
            mprs.insert("A".to_string(), tx);
            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    counter.fetch_add(1, Ordering::SeqCst);
                    println!("A receive");
                }
            });
        }

        {
            let (tx, mut rx) = mpsc::channel(10);
            mprs.insert("B".to_string(), tx);
            let counter = counter.clone();
            tokio::spawn(async move {
                while (rx.recv().await).is_some() {
                    counter.fetch_add(1, Ordering::SeqCst);
                    println!("B receive");
                }
            });
        }

        for _ in 0..10 {
            if let Some(sender) = mprs.get_random() {
                sender.1.try_send(12).unwrap();
            }
        }

        sleep(Duration::from_secs(10)).await;
        assert_eq!(10, counter.load(Ordering::SeqCst));
    }
}
