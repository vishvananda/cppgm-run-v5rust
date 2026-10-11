int tracker_alive;
int box_alive;
int conversions;
int invalid_lifetimes;

struct Tracker
{
  int value;
  Tracker(int input) : value(input) { ++tracker_alive; }
  Tracker(const Tracker & other) : value(other.value) { ++tracker_alive; }
  ~Tracker() { --tracker_alive; }
};

struct Box
{
  int value;
  Box(int input) : value(input) { ++box_alive; }
  Box(const Tracker & input) : value(input.value + 10)
  {
    if (tracker_alive != 1) ++invalid_lifetimes;
    ++box_alive;
    ++conversions;
  }
  Box(const Box & other) : value(other.value) { ++box_alive; }
  ~Box() { --box_alive; }
};

struct Reference
{
  Box * object;
  operator Box &() { return *object; }
};

struct Derived : Box
{
  Derived() : Box(11) {}
  Derived(const Box & input) : Box(input.value + 20) {}
};

int main()
{
  {
    Box existing(7);
    // A failed Tracker-to-Box& match must still consider Tracker-to-Box.
    Box left = false ? Tracker(5) : existing;
    Box right = true ? Tracker(5) : existing;
    Box reversed = false ? existing : Tracker(8);
    const Box fixed(3);
    Box constant = true ? Tracker(2) : fixed;
    if (left.value != 7 || right.value != 15 || reversed.value != 18 ||
        constant.value != 12 || tracker_alive != 0 || box_alive != 6 ||
        conversions != 3 || invalid_lifetimes != 0) return 1;
    Reference reference = { &existing };
    Box & selected = true ? reference : existing;
    selected.value = 9;
    if (&selected != &existing || existing.value != 9 || conversions != 3)
      return 2;
  }
  {
    Box base(5);
    Derived derived;
    Box & selected = false ? base : derived;
    // A reverse value conversion through Derived(Box const&) cannot compete
    // with this direct derived-to-base reference match.
    if (&selected != static_cast<Box *>(&derived) || selected.value != 11)
      return 3;
  }
  return tracker_alive != 0 || box_alive != 0 || invalid_lifetimes != 0;
}
