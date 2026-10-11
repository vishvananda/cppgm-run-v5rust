struct Value
{
  Value() : state(1) {}
  Value(const Value & other) : state(other.state + 1) {}
  Value(Value && other) : state(other.state + 2) { other.state = 0; }
  Value & operator=(const Value & other) { state = other.state + 3; return *this; }
  Value & operator=(Value && other) { state = other.state + 4; other.state = 0; return *this; }
  int state;
};

struct Owner
{
  Value values[1];
};

int main()
{
  Owner source;
  Owner copied(source);
  Owner moved(static_cast<Owner &&>(source));
  Owner copy_assigned;
  copy_assigned = copied;
  Owner move_assigned;
  move_assigned = static_cast<Owner &&>(copied);
  return source.values[0].state == 0 && moved.values[0].state == 3 &&
         copied.values[0].state == 0 && copy_assigned.values[0].state == 5 &&
         move_assigned.values[0].state == 6 ? 0 : 1;
}
