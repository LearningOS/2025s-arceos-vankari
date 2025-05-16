use alloc::vec::Vec;  
use core::borrow::Borrow;  
use core::fmt;  
use core::hash::{Hash, Hasher};  
use core::marker::PhantomData;  
use core::mem;  
use core::ops::{Index, IndexMut};  
use alloc::boxed::Box;
  
// 默认的初始容量  
const DEFAULT_CAPACITY: usize = 16;  
// 默认的负载因子，当元素数量超过容量*负载因子时扩容  
const DEFAULT_LOAD_FACTOR: f32 = 0.75;  
  
// 哈希函数，使用FNV-1a算法  
fn hash<T: Hash>(value: &T) -> u64 {  
    let mut hasher = FnvHasher::default();  
    value.hash(&mut hasher);  
    hasher.finish()  
}  
  
// FNV-1a哈希算法实现  
#[derive(Default)]  
struct FnvHasher {  
    hash: u64,  
}  
  
impl FnvHasher {  
    fn new() -> Self {  
        FnvHasher { hash: 0xcbf29ce484222325 }  
    }  
}  
  
impl Hasher for FnvHasher {  
    fn finish(&self) -> u64 {  
        self.hash  
    }  
  
    fn write(&mut self, bytes: &[u8]) {  
        for &byte in bytes {  
            self.hash ^= byte as u64;  
            self.hash = self.hash.wrapping_mul(0x100000001b3);  
        }  
    }  
}  
  
// 哈希表中的条目  
struct Entry<K, V> {  
    key: K,  
    value: V,  
    next: Option<Box<Entry<K, V>>>,  
}  
  
// HashMap实现  
pub struct HashMap<K, V> {  
    // 存储桶数组  
    buckets: Vec<Option<Box<Entry<K, V>>>>,  
    // 元素数量  
    size: usize,  
    // 负载因子  
    load_factor: f32,  
    // 标记类型参数  
    _marker: PhantomData<(K, V)>,  
}  
  
impl<K, V> HashMap<K, V>  
where  
    K: Eq + Hash,  
{  
    // 创建一个新的HashMap  
    pub fn new() -> Self {  
        Self::with_capacity(DEFAULT_CAPACITY)  
    }  
  
    // 创建一个指定容量的HashMap  
    pub fn with_capacity(capacity: usize) -> Self {  
        let mut buckets = Vec::with_capacity(capacity);  
        buckets.resize_with(capacity, || None);  
          
        HashMap {  
            buckets,  
            size: 0,  
            load_factor: DEFAULT_LOAD_FACTOR,  
            _marker: PhantomData,  
        }  
    }  
  
    // 获取元素数量  
    pub fn len(&self) -> usize {  
        self.size  
    }  
  
    // 判断是否为空  
    pub fn is_empty(&self) -> bool {  
        self.size == 0  
    }  
  
    // 清空HashMap  
    pub fn clear(&mut self) {  
        for bucket in &mut self.buckets {  
            *bucket = None;  
        }  
        self.size = 0;  
    }  
  
    // 计算键的桶索引  
    fn bucket_index(&self, key: &K) -> usize {  
        let hash_code = hash(key);  
        (hash_code as usize) % self.buckets.len()  
    }  
  
    // 插入键值对  
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {  
        // 检查是否需要扩容  
        if (self.size as f32) >= (self.buckets.len() as f32 * self.load_factor) {  
            self.resize(self.buckets.len() * 2);  
        }  
  
        let index = self.bucket_index(&key);  
          
        // 如果桶为空，直接插入  
        if self.buckets[index].is_none() {  
            self.buckets[index] = Some(Box::new(Entry {  
                key,  
                value,  
                next: None,  
            }));  
            self.size += 1;  
            return None;  
        }  
          
        // 处理链表  
        let mut current = &mut self.buckets[index];  
          
        while let Some(entry) = current {  
            // 如果键已存在，更新值  
            if entry.key == key {  
                let old_value = mem::replace(&mut entry.value, value);  
                return Some(old_value);  
            }  
              
            // 移动到下一个节点  
            if entry.next.is_none() {  
                // 到达链表末尾，添加新节点  
                entry.next = Some(Box::new(Entry {  
                    key,  
                    value,  
                    next: None,  
                }));  
                self.size += 1;  
                return None;  
            }  
              
            current = &mut entry.next;  
        }  
          
        unreachable!()  
    }  
  
    // 获取键对应的值的引用  
    pub fn get<Q>(&self, key: &Q) -> Option<&V>  
    where  
        K: Borrow<Q>,  
        Q: Hash + Eq + ?Sized,  
    {  
        let index = {  
            let hash_code = {  
                let mut hasher = FnvHasher::new();  
                key.hash(&mut hasher);  
                hasher.finish()  
            };  
            (hash_code as usize) % self.buckets.len()  
        };  
          
        let mut current = &self.buckets[index];  
          
        while let Some(entry) = current {  
            if entry.key.borrow() == key {  
                return Some(&entry.value);  
            }  
            current = &entry.next;  
        }  
          
        None  
    }  
  
    // 获取键对应的值的可变引用  
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>  
    where  
        K: Borrow<Q>,  
        Q: Hash + Eq + ?Sized,  
    {  
        let index = {  
            let hash_code = {  
                let mut hasher = FnvHasher::new();  
                key.hash(&mut hasher);  
                hasher.finish()  
            };  
            (hash_code as usize) % self.buckets.len()  
        };  
          
        let mut current = &mut self.buckets[index];  
          
        while let Some(entry) = current {  
            if entry.key.borrow() == key {  
                return Some(&mut entry.value);  
            }  
            current = &mut entry.next;  
        }  
          
        None  
    }  
  
    // 移除键对应的值  
    pub fn remove<Q>(&mut self, key: &Q) -> Option<V>  
    where  
        K: Borrow<Q>,  
        Q: Hash + Eq + ?Sized,  
    {  
        let index = {  
            let hash_code = {  
                let mut hasher = FnvHasher::new();  
                key.hash(&mut hasher);  
                hasher.finish()  
            };  
            (hash_code as usize) % self.buckets.len()  
        };  
          
        let bucket = &mut self.buckets[index];  
          
        if bucket.is_none() {  
            return None;  
        }  
          
        // 特殊处理第一个节点  
        if bucket.as_ref().unwrap().key.borrow() == key {  
            let entry = bucket.take().unwrap();  
            *bucket = entry.next;  
            self.size -= 1;  
            return Some(entry.value);  
        }  
          
        // 处理链表中的节点  
        let mut current = bucket;  
          
        while let Some(entry) = current {  
            if let Some(ref next) = entry.next {  
                if next.key.borrow() == key {  
                    let mut removed = entry.next.take().unwrap();  
                    entry.next = removed.next.take();  
                    self.size -= 1;  
                    return Some(removed.value);  
                }  
            }  
              
            if let Some(ref mut next) = entry.next {  
                current = &mut entry.next;  
            } else {  
                break;  
            }  
        }  
          
        None  
    }  
  
    // 检查是否包含键  
    pub fn contains_key<Q>(&self, key: &Q) -> bool  
    where  
        K: Borrow<Q>,  
        Q: Hash + Eq + ?Sized,  
    {  
        self.get(key).is_some()  
    }  
  
    // 扩容  
    fn resize(&mut self, new_capacity: usize) {  
        let mut new_buckets = Vec::with_capacity(new_capacity);  
        new_buckets.resize_with(new_capacity, || None);  
      
        let old_buckets = mem::replace(&mut self.buckets, new_buckets);  
        self.size = 0;  
      
        for bucket in old_buckets.into_iter().flatten() {  
            let mut entry = bucket;  
            loop {  
                // Wrap in ManuallyDrop to prevent dropping the moved values  
                let mut key = mem::ManuallyDrop::new(entry.key);  
                let mut value = mem::ManuallyDrop::new(entry.value);  
              
                // Take ownership without dropping  
                let key_owned = unsafe { mem::ManuallyDrop::take(&mut key) };  
                let value_owned = unsafe { mem::ManuallyDrop::take(&mut value) };  
              
                self.insert(key_owned, value_owned);  
              
                if let Some(next) = entry.next.take() {  
                    entry = next;  
                } else {  
                    break;  
                }  
            }  
        }  
    }
    // 创建迭代器  
    pub fn iter(&self) -> Iter<K, V> {  
        let mut bucket_index = 0;  
        let mut current_entry = None;  
          
        // 找到第一个非空桶  
        while bucket_index < self.buckets.len() {  
            if let Some(ref entry) = self.buckets[bucket_index] {  
                current_entry = Some(&**entry);  
                break;  
            }  
            bucket_index += 1;  
        }  
          
        Iter {  
            map: self,  
            bucket_index,  
            current_entry,  
        }  
    }  
}  
  
// 实现默认trait  
impl<K, V> Default for HashMap<K, V>  
where  
    K: Eq + Hash,  
{  
    fn default() -> Self {  
        Self::new()  
    }  
}  
  
// 实现索引trait  
impl<K, V, Q> Index<&Q> for HashMap<K, V>  
where  
    K: Borrow<Q> + Eq + Hash,  
    Q: Hash + Eq + ?Sized,  
{  
    type Output = V;  
      
    fn index(&self, key: &Q) -> &Self::Output {  
        self.get(key).expect("no entry found for key")  
    }  
}  
  
impl<K, V, Q> IndexMut<&Q> for HashMap<K, V>  
where  
    K: Borrow<Q> + Eq + Hash,  
    Q: Hash + Eq + ?Sized,  
{  
    fn index_mut(&mut self, key: &Q) -> &mut Self::Output {  
        self.get_mut(key).expect("no entry found for key")  
    }  
}  
  
// 实现Debug trait  
impl<K, V> fmt::Debug for HashMap<K, V>  
where  
    K: fmt::Debug + Eq + Hash,  
    V: fmt::Debug,  
{  
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {  
        f.debug_map().entries(self.iter()).finish()  
    }  
}  
  
// 迭代器实现  
pub struct Iter<'a, K, V> {  
    map: &'a HashMap<K, V>,  
    bucket_index: usize,  
    current_entry: Option<&'a Entry<K, V>>,  
}  
  
impl<'a, K, V> Iterator for Iter<'a, K, V> {  
    type Item = (&'a K, &'a V);  
      
    fn next(&mut self) -> Option<Self::Item> {  
        if let Some(entry) = self.current_entry {  
            // 保存当前条目的引用  
            let result = (&entry.key, &entry.value);  
              
            // 移动到下一个条目  
            if let Some(ref next) = entry.next {  
                self.current_entry = Some(&**next);  
            } else {  
                // 当前链表结束，移动到下一个桶  
                self.bucket_index += 1;  
                self.current_entry = None;  
                  
                while self.bucket_index < self.map.buckets.len() {  
                    if let Some(ref entry) = self.map.buckets[self.bucket_index] {  
                        self.current_entry = Some(&**entry);  
                        break;  
                    }  
                    self.bucket_index += 1;  
                }  
            }  
              
            Some(result)  
        } else {  
            None  
        }  
    }  
}