int boxes_alive;
int trackers_alive;

struct Tracker
{
  int value;
  Tracker(int input) : value(input) { ++trackers_alive; }
  ~Tracker() { --trackers_alive; }
};

struct Box
{
  int value;
  Box(int input) : value(input) { ++boxes_alive; }
  Box(const Tracker & input) : value(input.value + 10) { ++boxes_alive; }
  Box(const Box & other) : value(other.value) { ++boxes_alive; }
  Box(Box && other) : value(other.value) { ++boxes_alive; }
  ~Box() { --boxes_alive; }
};

int category(const Box &) { return 0; }
int category(Box &&) { return 1; }

const Box make_constant()
{
  return Box(8);
}

int main()
{
  {
    const Box fixed(7);
    if (category(false ? Box(3) : fixed) != 0) return 1;
    if (category(true ? Box(3) : fixed) != 0) return 1;
    if (category(true ? Tracker(5) : fixed) != 0) return 1;
    if (category(false ? make_constant() : Box(4)) != 0) return 1;
    if (category(false ? Box(3) : Box(4)) != 1) return 1;
    if (boxes_alive != 1 || trackers_alive != 0) return 2;
    const Box & extended = true ? Tracker(5) : fixed;
    if (extended.value != 15 || boxes_alive != 2 || trackers_alive != 0)
      return 3;
  }
  return boxes_alive != 0 || trackers_alive != 0;
}
