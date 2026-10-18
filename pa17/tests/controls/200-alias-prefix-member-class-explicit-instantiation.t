// expect exit 0
// An alias template in the qualifier can still name a real member class.
template<class T> struct Box {
  struct Member {
    static int size() { return sizeof(T); }
  };
};
template<class T> using Alias = Box<T>;
template struct Alias<int>::Member;
struct Alias<char>::Member* pointer;
int main() {
  return Alias<int>::Member::size() == sizeof(int) && pointer == 0 ? 0 : 1;
}
