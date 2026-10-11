namespace n {
template<class A, class B>
using same = int;

template<class T>
T declval();
}

namespace n {
template<class T>
struct box {
  typedef same<decltype(declval<T>()), int> type;
};
}

int main() { return 0; }
