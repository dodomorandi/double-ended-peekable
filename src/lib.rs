//! A peekable abstraction for double-ended iterators.
//!
//! This crate provides an _extension trait_ to standard [`Iterator`] in order to lift the
//! [`Peekable`] abstraction over types implementing [`DoubleEndedIterator`].
//!
//! # Basic usage
//!
//! ```
//! use double_ended_peekable::DoubleEndedPeekableExt;
//!
//! // Now you can use `iter.double_ended_peekable()`
//! let mut iter = [1, 2, 3, 4].into_iter().double_ended_peekable();
//! // Same abstractions of `Peekable`
//! assert_eq!(iter.peek(), Some(&1));
//! // Additional `*_back*` methods
//! assert_eq!(iter.peek_back(), Some(&4));
//! // It implements `Iterator`
//! assert_eq!(iter.next(), Some(1));
//! // And also `DoubleEndedIterator` when possible
//! assert_eq!(iter.next_back(), Some(4));
//! // Additional methods for both front and back items
//! assert_eq!(iter.next_front_back_if_eq(&2, &3), Some((2, 3)));
//! ```
//!
//! Check [`DoubleEndedPeekable`] documentation for additional information.
//!
//! # Rationale
//!
//! It is possible to use [`Peekable`] on double-ended iterators using `.rev().peekable()`:
//!
//! ```
//! let mut iter = [1, 2, 3].into_iter().rev().peekable();
//! // No problem!
//! assert_eq!(iter.peek(), Some(&3));
//! ````
//!
//! However, using `.by_ref().rev().peekable()` _on the fly_ is a footgun:
//! ```should_panic
//! let mut iter = [1, 2, 3, 4].into_iter().peekable();
//! assert_eq!(iter.peek(), Some(&1));
//! assert_eq!(iter.by_ref().rev().peekable().peek(), Some(&4));
//! assert_eq!(iter.next(), Some(1));
//!
//! // Dang! This fails: iter.next_back() == Some(3)
//! assert_eq!(iter.next_back(), Some(4));
//! ```
//!
//! The reason this happen is simply because [`Peekable`] saves the next item of the iterator
//! internally, therefore creating a _rev-peekable_ iterator on the fly is risky because there is a
//! good chance a peeked element is going to be accidentally lost.
//!
//! This tiny crate a simple but powerful abstraction that is hard to misuse.
//!
//! [`Peekable`]: core::iter::Peekable

#![cfg_attr(not(test), no_std)]

#[cfg(test)]
mod tests;

use core::{
    fmt::{self, Debug},
    hash::{Hash, Hasher},
};

pub trait DoubleEndedPeekableExt<I: Iterator> {
    fn double_ended_peekable(self) -> DoubleEndedPeekable<I>;
}

impl<I> DoubleEndedPeekableExt<I> for I
where
    I: Iterator,
{
    #[inline]
    fn double_ended_peekable(self) -> DoubleEndedPeekable<I> {
        DoubleEndedPeekable {
            iter: self,
            front: None,
            back: None,
        }
    }
}

pub struct DoubleEndedPeekable<I: Iterator> {
    iter: I,
    front: Option<Option<<I as Iterator>::Item>>,
    back: Option<Option<<I as Iterator>::Item>>,
}

impl<I: Iterator> DoubleEndedPeekable<I> {
    #[inline]
    pub fn peek(&mut self) -> Option<&I::Item> {
        self.front
            .get_or_insert_with(|| self.iter.next())
            .as_ref()
            .or_else(|| self.back.as_ref().and_then(Option::as_ref))
    }

    #[inline]
    pub fn peek_mut(&mut self) -> Option<&mut I::Item> {
        self.front
            .get_or_insert_with(|| self.iter.next())
            .as_mut()
            .or_else(|| self.back.as_mut().and_then(Option::as_mut))
    }

    #[inline]
    pub fn next_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next() {
            Some(item) if func(&item) => Some(item),
            other => {
                debug_assert!(self.front.is_none());
                self.front = Some(other);
                None
            }
        }
    }

    #[inline]
    pub fn next_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_if(|item| item == expected)
    }
}

impl<I: DoubleEndedIterator> DoubleEndedPeekable<I> {
    #[inline]
    pub fn peek_back(&mut self) -> Option<&I::Item> {
        self.back
            .get_or_insert_with(|| self.iter.next_back())
            .as_ref()
            .or_else(|| self.front.as_ref().and_then(Option::as_ref))
    }

    #[inline]
    pub fn peek_back_mut(&mut self) -> Option<&mut I::Item> {
        self.back
            .get_or_insert_with(|| self.iter.next_back())
            .as_mut()
            .or_else(|| self.front.as_mut().and_then(Option::as_mut))
    }

    #[inline]
    pub fn next_back_if(&mut self, func: impl FnOnce(&I::Item) -> bool) -> Option<I::Item> {
        match self.next_back() {
            Some(item) if func(&item) => Some(item),
            other => {
                debug_assert!(self.back.is_none());
                self.back = Some(other);
                None
            }
        }
    }

    #[inline]
    pub fn next_back_if_eq<T>(&mut self, expected: &T) -> Option<I::Item>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_back_if(|item| item == expected)
    }

    #[inline]
    pub fn next_front_back_if(
        &mut self,
        func: impl FnOnce(&I::Item, &I::Item) -> bool,
    ) -> Option<(I::Item, I::Item)> {
        match (self.next(), self.next_back()) {
            (Some(front), Some(back)) if func(&front, &back) => Some((front, back)),
            (front, back) => {
                debug_assert!(self.front.is_none());
                debug_assert!(self.back.is_none());
                self.front = Some(front);
                self.back = Some(back);
                None
            }
        }
    }

    #[inline]
    pub fn next_front_back_if_eq<T>(
        &mut self,
        expected_front: &T,
        expected_back: &T,
    ) -> Option<(I::Item, I::Item)>
    where
        T: ?Sized,
        I::Item: PartialEq<T>,
    {
        self.next_front_back_if(|front, back| front == expected_front && back == expected_back)
    }
}

impl<I> Iterator for DoubleEndedPeekable<I>
where
    I: Iterator,
{
    type Item = I::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self.front.take() {
            Some(out @ Some(_)) => out,
            Some(None) => self.back.take().flatten(),
            None => match self.iter.next() {
                item @ Some(_) => item,
                None => self.back.take().flatten(),
            },
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.iter.size_hint();
        let additional = match (&self.front, &self.back) {
            (Some(_), Some(_)) => 2,
            (Some(_), _) | (_, Some(_)) => 1,
            (None, None) => 0,
        };

        (lower + additional, upper.map(|upper| upper + additional))
    }
}

impl<I> DoubleEndedIterator for DoubleEndedPeekable<I>
where
    I: DoubleEndedIterator,
{
    #[inline]
    fn next_back(&mut self) -> Option<Self::Item> {
        match self.back.take() {
            Some(out @ Some(_)) => out,
            Some(None) => self.front.take().flatten(),
            None => match self.iter.next_back() {
                out @ Some(_) => out,
                None => self.front.take().flatten(),
            },
        }
    }
}

impl<I> Debug for DoubleEndedPeekable<I>
where
    I: Iterator + Debug,
    I::Item: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DoubleEndedPeekable")
            .field("iter", &self.iter)
            .field("front", &self.front)
            .field("back", &self.back)
            .finish()
    }
}

impl<I> Clone for DoubleEndedPeekable<I>
where
    I: Iterator + Clone,
    I::Item: Clone,
{
    #[inline]
    fn clone(&self) -> Self {
        Self {
            iter: self.iter.clone(),
            front: self.front.clone(),
            back: self.back.clone(),
        }
    }
}

impl<I> Copy for DoubleEndedPeekable<I>
where
    I: Iterator + Copy,
    I::Item: Copy,
{
}

impl<I> PartialEq for DoubleEndedPeekable<I>
where
    I: Iterator + PartialEq,
    I::Item: PartialEq,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.iter == other.iter && self.front == other.front && self.back == other.back
    }
}

impl<I> Eq for DoubleEndedPeekable<I>
where
    I: Iterator + Eq,
    I::Item: Eq,
{
}

impl<I> Hash for DoubleEndedPeekable<I>
where
    I: Iterator + Hash,
    I::Item: Hash,
{
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.iter.hash(state);
        self.front.hash(state);
        self.back.hash(state);
    }
}
