use std::{
    ops::{Range, RangeFrom, RangeFull, RangeTo},
    str::{CharIndices, Chars, FromStr},
};

use nom::{
    AsBytes, Compare, Err, ExtendInto, FindSubstring, FindToken, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Offset, ParseTo, Slice,
};

#[derive(Clone, Debug, Copy)]
pub struct Input<'a> {
    pub data: &'a str,
}

impl<'a> Input<'a> {
    pub fn new(data: &'a str) -> Self {
        Self { data }
    }
    pub fn location_line(&self) -> usize {
        0
    }

    pub fn get_utf8_column(&self) -> usize {
        10
    }

    pub fn fragment(&self) -> &'a str {
        self.data
    }

    pub fn starts_with(&self, char: char) -> bool {
        self.data.starts_with(char)
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<'a> AsBytes for Input<'a> {
    fn as_bytes(&self) -> &[u8] {
        self.data.as_bytes()
    }
}

impl<'a, 'b> Compare<&'b str> for Input<'a> {
    fn compare(&self, t: &'b str) -> nom::CompareResult {
        self.data.compare(t)
    }

    fn compare_no_case(&self, t: &'b str) -> nom::CompareResult {
        self.data.compare_no_case(t)
    }
}

impl<'a> ExtendInto for Input<'a> {
    type Item = char;

    type Extender = String;

    fn new_builder(&self) -> Self::Extender {
        String::new()
    }

    fn extend_into(&self, acc: &mut Self::Extender) {
        acc.push_str(self.data);
    }
}

impl<'a, 'b> FindSubstring<&'b str> for Input<'a> {
    fn find_substring(&self, substr: &'b str) -> Option<usize> {
        self.data.find_substring(substr)
    }
}

impl<'a> FindToken<u8> for Input<'a> {
    fn find_token(&self, token: u8) -> bool {
        self.data.find_token(token)
    }
}

impl<'a> InputIter for Input<'a> {
    type Item = char;

    type Iter = CharIndices<'a>;

    type IterElem = Chars<'a>;

    fn iter_indices(&self) -> Self::Iter {
        self.data.iter_indices()
    }

    fn iter_elements(&self) -> Self::IterElem {
        self.data.iter_elements()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
    where
        P: Fn(Self::Item) -> bool,
    {
        self.data.position(predicate)
    }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        self.data.slice_index(count)
    }
}

impl<'a> InputLength for Input<'a> {
    fn input_len(&self) -> usize {
        self.data.len()
    }
}

impl<'a> InputTake for Input<'a> {
    fn take(&self, count: usize) -> Self {
        self.slice(..count)
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        (self.slice(count..), self.slice(..count))
    }
}

impl<'a> InputTakeAtPosition for Input<'a> {
    type Item = char;

    fn split_at_position<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.data.position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(nom::Needed::new(1))),
        }
    }

    fn split_at_position1<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
        e: nom::error::ErrorKind,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.data.position(predicate) {
            Some(n) => Ok(self.take_split(n)),
            None => Err(Err::Incomplete(nom::Needed::new(1))),
        }
    }

    fn split_at_position_complete<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.split_at_position(predicate) {
            Err(Err::Incomplete(_)) => Ok(self.take_split(self.input_len())),
            res => res,
        }
    }

    fn split_at_position1_complete<P, E: nom::error::ParseError<Self>>(
        &self,
        predicate: P,
        e: nom::error::ErrorKind,
    ) -> nom::IResult<Self, Self, E>
    where
        P: Fn(Self::Item) -> bool,
    {
        match self.data.position(predicate) {
            Some(0) => Err(Err::Error(E::from_error_kind(self.clone(), e))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.data.input_len() == 0 {
                    Err(Err::Error(E::from_error_kind(self.clone(), e)))
                } else {
                    Ok(self.take_split(self.input_len()))
                }
            }
        }
    }
}

impl<'a> Offset for Input<'a> {
    fn offset(&self, second: &Self) -> usize {
        self.data.offset(second.data)
    }
}

impl<'a, R: FromStr> ParseTo<R> for Input<'a> {
    fn parse_to(&self) -> Option<R> {
        self.data.parse_to()
    }
}

impl<'a> Slice<Range<usize>> for Input<'a> {
    fn slice(&self, range: Range<usize>) -> Self {
        Self {
            data: self.data.slice(range),
        }
    }
}

impl<'a> Slice<RangeTo<usize>> for Input<'a> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        Self {
            data: self.data.slice(range),
        }
    }
}
impl<'a> Slice<RangeFrom<usize>> for Input<'a> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        Self {
            data: self.data.slice(range),
        }
    }
}
impl<'a> Slice<RangeFull> for Input<'a> {
    fn slice(&self, range: RangeFull) -> Self {
        Self {
            data: self.data.slice(range),
        }
    }
}
