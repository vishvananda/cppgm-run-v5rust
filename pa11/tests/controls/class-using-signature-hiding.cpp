// expect exit 0
struct Base {
  int select(int) { return 1; }
  int select(long) { return 3; }
  int select(int) const { return 5; }
  int operator+(int) { return 7; }
  int operator+(long) { return 9; }
};
struct After : Base {
  using Base::select;
  using Base::operator+;
  int select(int) { return 2; }
  int operator+(int) { return 8; }
  using Base::select;
  using Base::operator+;
};
struct Before : Base {
  int select(int) { return 4; }
  using Base::select;
};
int main() {
  After after;
  Before before;
  const After& constant = after;
  return after.select(0) == 2 && after.select(0L) == 3 &&
         constant.select(0) == 5 && before.select(0) == 4 &&
         before.select(0L) == 3 && after + 0 == 8 &&
         after.Base::operator+(0L) == 9
             ? 0 : 1;
}
