// A move-only aggregate member can be initialized from a class prvalue.
// Construction may reuse the final member address under the copy-elision rules.
struct move_only
{
  int value;

  move_only(int v) : value(v)
  {
  }

  move_only(move_only && other) : value(other.value)
  {
    other.value = 0;
  }

  move_only(move_only const &) = delete;
};

struct aggregate
{
  int prefix;
  move_only member;
};

aggregate make_value()
{
  return aggregate{3, move_only(7)};
}

int main()
{
  aggregate value = make_value();
  return value.prefix == 3 && value.member.value == 7 ? 0 : 1;
}
