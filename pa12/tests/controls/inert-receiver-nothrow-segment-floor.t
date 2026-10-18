// expect exit 0
// An inert receiver ends the initial nonthrowing region. Outer automatic
// objects below the segment floor do not make an active temporary chain.
int destructions;
struct Guard {
  ~Guard() noexcept { ++destructions; }
};
struct Result {
  ~Result() noexcept { ++destructions; }
  void run() noexcept {}
};
struct Source {
  Result get() noexcept { return Result(); }
};
Source make_source() noexcept { return Source(); }
void exercise() {
  Guard outer;
  make_source().get().run();
}
int main() {
  exercise();
  return destructions != 2;
}
