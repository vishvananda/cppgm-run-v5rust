// VALIDATION: compile-pass
// N3485 focus: 14.5.5 [temp.class.spec], 14.6.2 [temp.dep]

template<bool> struct flag {};
template<class T> struct ref { typedef T& type; };
template<class A, class B> struct pair { typedef pair type; };

template<int N, class It, class State>
struct step : step<N - 1, It, pair<typename State::type, It const> > {};

template<class It, class State>
struct step<0, It, State> { typedef typename State::type type; };

typedef step<1, int, ref<flag<false> > >::type answer;

int main()
{
  return 0;
}
