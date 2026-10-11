// N3485 5.2.5p4, 9.3.2p3: inside a volatile member function `this` points
// to a volatile object; `mutable` removes only const, so a mutable member
// read through that object is still a volatile access.
struct Counter
{
  mutable int reads;
  int value;
  int get() const volatile { reads = reads + 1; return value; }
  void set(int next) volatile { value = next; }
};

int main()
{
  volatile Counter counter;
  counter.reads = 0;
  counter.set(4);
  return counter.get() == 4 && counter.reads == 1 ? 0 : 1;
}
