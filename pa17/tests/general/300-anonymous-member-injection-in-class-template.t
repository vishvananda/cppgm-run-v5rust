// An anonymous class member injects its own members into the enclosing class,
// so it has to be complete before they can be read.  At namespace scope it
// already is by that point; nested inside a class template it is not, and
// reading an empty member list injects nothing and reports nothing -- every
// later use of an injected name then fails to find it.
//
// libc++ writes basic_string's __long and __short this way, so the failure
// reaches anything that asks whether a string is long.

typedef unsigned long size_type;

template<class C>
struct str
{
  struct storage
  {
    struct
    {
      size_type is_long_;
      size_type cap_;
    };
    size_type size_;
  };

  struct inline_storage
  {
    struct
    {
      size_type is_long_;
    };
    C data_[8];
  };

  union rep { storage long_; inline_storage short_; };

  rep rep_;

  // Naming each injected member is the assertion: without the injection none
  // of them resolves.  The non-anonymous sibling must stay reachable too.
  size_type total() const
  {
    return rep_.long_.is_long_ + rep_.long_.cap_ + rep_.long_.size_;
  }
};

int main()
{
  str<char> made = str<char>();
  const size_type through_member = made.total();
  const size_type through_object =
    made.rep_.long_.is_long_ + made.rep_.long_.cap_ + made.rep_.long_.size_;
  return (through_member == 0 && through_object == 0) ? 0 : 1;
}
