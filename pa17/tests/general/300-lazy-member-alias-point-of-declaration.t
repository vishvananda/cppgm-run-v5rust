// VALIDATION: compile-pass
// Lazy member lookup must not let a later typedef shadow a namespace template
// while resolving an earlier member's type.

namespace n {

template<class S> struct end {
  typedef typename S::end type;
};

namespace aux {
template<class A> struct iter {};
template<class A> struct next {
  typedef iter<A> type;
};
}

template<class Sequence> struct view {
private:
  typedef typename end<Sequence>::type last_;
public:
  typedef typename aux::next<last_>::type begin;
  typedef aux::iter<last_> end;
};

}

struct input {
  typedef int end;
};

int main() {
  typedef n::view<input> filtered;
  static_assert(sizeof(filtered::begin) == sizeof(filtered::end), "");
  return 0;
}
