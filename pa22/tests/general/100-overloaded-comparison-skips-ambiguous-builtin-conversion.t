// VALIDATION: compile-pass
// N3485 focus: 13.3.1.2 [over.match.oper], 12.3.2 [class.conv.fct]

struct marker
{
};

struct deque_sequence_base
{
  operator marker() const
  {
    return marker();
  }
};

struct extended_sequence_base
{
  operator marker() const
  {
    return marker();
  }
};

struct deque : deque_sequence_base
{
};

struct keyed_element : deque
{
};

struct front_extended_deque : keyed_element, extended_sequence_base
{
};

bool operator==(const front_extended_deque &, const deque &)
{
  return true;
}

int test(bool v)
{
  return v ? 0 : 1;
}

int main()
{
  front_extended_deque extended;
  deque other;
  return test(extended == other);
}
