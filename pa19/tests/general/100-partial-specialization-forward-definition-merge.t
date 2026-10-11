template<class T, class Alloc>
struct vector;

template<class Alloc>
struct vector<bool, Alloc>;

template<class Alloc>
struct vector<bool, Alloc> {
  void f() noexcept;
};

template<class Alloc>
void vector<bool, Alloc>::f() noexcept {}

int main() {
  return 0;
}
