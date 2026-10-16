// N3485 14.7.2: an explicit member root is independent of ordinary use; expect exit 0
struct NoCopy {
  int n;
  NoCopy(const NoCopy&) = delete;
};
template<class T> struct Box {
  T value;
  Box(const Box&) = default;
};
template struct Box<NoCopy>;
template<class T> struct BaseBox : T {
  int n;
  BaseBox(const BaseBox&) = default;
};
template struct BaseBox<NoCopy>;
struct UserCopy { int n; UserCopy(const UserCopy&) noexcept {} };
template<class T> struct MixedBox {
  T first;
  UserCopy second;
  MixedBox(const MixedBox&) = default;
};
template struct MixedBox<NoCopy>;
template<class T> struct Assign {
  T value;
  Assign& operator=(const Assign&) = default;
};
template struct Assign<int>;
int main() { return 0; }
