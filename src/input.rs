use bytecount::num_chars;
use memchr::Memchr;
use nom::{
    AsBytes, Compare, Err, ExtendInto, FindSubstring, FindToken, InputIter, InputLength, InputTake,
    InputTakeAtPosition, Offset, ParseTo, Slice,
};
use std::{
    ops::{Range, RangeFrom, RangeTo},
    str::{CharIndices, Chars, FromStr},
};

#[derive(Clone, Debug, Copy)]
pub struct Input<'a> {
    pub data: &'a str,
    line: usize,
    col: usize,
}

impl<'a> Input<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            line: 1,
            col: 1,
        }
    }
    pub fn location_line(&self) -> usize {
        self.line
    }

    pub fn get_utf8_column(&self) -> usize {
        self.col
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
        _e: nom::error::ErrorKind,
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
            Some(0) => Err(Err::Error(E::from_error_kind(*self, e))),
            Some(n) => Ok(self.take_split(n)),
            None => {
                if self.data.input_len() == 0 {
                    Err(Err::Error(E::from_error_kind(*self, e)))
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
        let next_data = self.data.slice(range);

        Self {
            data: next_data,
            line: 0,
            col: 1,
        }
    }
}

impl<'a> Input<'a> {
    fn slice_common(&self, next_data: &'a str) -> Self {
        let offset = self.data.offset(next_data);

        let old_data = self.data.slice(..offset);

        if offset == 0 {
            return Self {
                data: next_data,
                line: self.line,
                col: self.col,
            };
        }

        let new_line_iter = Memchr::new(b'\n', old_data.as_bytes());

        let mut lines_to_add = 0;
        let mut last_index = None;
        for i in new_line_iter {
            lines_to_add += 1;
            last_index = Some(i);
        }
        let last_index = last_index.map(|v| v + 1).unwrap_or(0);

        let col = num_chars(old_data.as_bytes().slice(last_index..));

        Self {
            data: next_data,
            line: self.line + lines_to_add,
            col: if lines_to_add == 0 {
                self.col + col
            } else {
                // When going to a new line, char starts at 1
                col + 1
            },
        }
    }
}

impl<'a> Slice<RangeTo<usize>> for Input<'a> {
    fn slice(&self, range: RangeTo<usize>) -> Self {
        let next_data = self.data.slice(range);

        self.slice_common(next_data)
    }
}
impl<'a> Slice<RangeFrom<usize>> for Input<'a> {
    fn slice(&self, range: RangeFrom<usize>) -> Self {
        let next_data = self.data.slice(range);

        self.slice_common(next_data)
    }
}
