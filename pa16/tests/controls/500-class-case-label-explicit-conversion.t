// N3485 5.19p3, 6.4.2p2, 12.3.2: expect reject
struct Label {
  explicit constexpr operator int() const { return 1; }
};
int pick(int n) {
  switch (n) { case Label(): return 1; }
  return 0;
}
