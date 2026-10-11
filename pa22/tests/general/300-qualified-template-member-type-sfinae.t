template<class T>
struct member_pointer_class_type {};

template<class R, class C>
struct member_pointer_class_type<R C::*> {
  typedef C type;
};

struct yes {
  char c[1];
};

struct no {
  char c[2];
};

template<class F,
         class DecayF = F,
         class ClassT = typename member_pointer_class_type<DecayF>::type>
yes probe(int);

template<class F>
no probe(...);

struct Functor {};

int check[sizeof(probe<Functor>(0)) == sizeof(no) ? 1 : -1];

int main() {}
