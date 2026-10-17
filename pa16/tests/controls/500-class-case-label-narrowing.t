// N3485 5.19p3, 6.4.2p2: expect reject
struct Label {
  constexpr operator long() const { return 0x100000000L; }
};
int pick(int n) {
  switch (n) { case Label(): return 1; }
  return 0;
}
