// VALIDATION: compile-fail
// A demanded member type instantiates the preceding address-bearing typedef;
// validating that function address must materialize and check the destructor.
template<void (*)()> struct instantiate {};
template<class M> struct requirement {
  static void failed() { ((M *)0)->~M(); }
};
template<class T> struct model {
  typedef instantiate<&requirement<model>::failed> check;
  ~model() { T value = "bad"; }
  typedef int associated;
};
typedef model<int>::associated trigger;
