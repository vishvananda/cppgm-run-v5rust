// Empty member padding is zero before its constructor; an empty base overlaps.
void* operator new(unsigned long, void* p) noexcept { return p; }
int observed, count;
struct Member {
  Member() { observed = *reinterpret_cast<unsigned char*>(this); ++count; }
  ~Member() { --count; }
};
struct Iterator { Member value; };
struct Empty {};
struct Base : Empty { int value; Base() : Empty(), value(17) {} };
struct User { unsigned char value; User() { value = 23; } };
struct Padded { char a; int b; };
struct Bits { unsigned a : 3; unsigned b : 5; };
union Choice { char first; long second; };
int check_padding() {
  alignas(Padded) unsigned char padded[sizeof(Padded)];
  alignas(Bits) unsigned char bits[sizeof(Bits)];
  alignas(Choice) unsigned char choice[sizeof(Choice)];
  alignas(Empty) unsigned char empty[sizeof(Empty)];
  for (unsigned i = 0; i < sizeof(Padded); ++i) padded[i] = 165;
  for (unsigned i = 0; i < sizeof(Bits); ++i) bits[i] = 165;
  for (unsigned i = 0; i < sizeof(Choice); ++i) choice[i] = 165;
  empty[0] = 165;
  Padded* p = new(padded) Padded();
  Bits* b = new(bits) Bits();
  Choice* c = new(choice) Choice();
  Empty* e = new(empty) Empty();
  for (unsigned i = 0; i < sizeof(Padded); ++i)
    if (reinterpret_cast<unsigned char*>(p)[i]) return 1;
  for (unsigned i = 0; i < sizeof(Bits); ++i)
    if (reinterpret_cast<unsigned char*>(b)[i]) return 2;
  for (unsigned i = 0; i < sizeof(Choice); ++i)
    if (reinterpret_cast<unsigned char*>(c)[i]) return 3;
  return *reinterpret_cast<unsigned char*>(e) != 0;
}
int main() {
  alignas(Iterator) unsigned char bytes[sizeof(Iterator)];
  for (unsigned i = 0; i < sizeof(Iterator); ++i) bytes[i] = 165;
  Iterator* p = new(bytes) Iterator();
  int result = observed != 0 ||
    *reinterpret_cast<unsigned char*>(p) != 0 || count != 1;
  p->~Iterator();
  bytes[0] = 165;
  p = new(bytes) Iterator;
  result |= observed != 165;
  p->~Iterator();
  bytes[0] = 165;
  p = new(bytes) Iterator{};
  result |= observed != 165;
  p->~Iterator();
  Base base;
  User user = User();
  return result || count || base.value != 17 || user.value != 23 || check_padding();
}
