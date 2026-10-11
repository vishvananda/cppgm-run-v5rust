// A member-pointer non-type argument retains its exact typed identity through
// constructor-template deduction. A named non-type template parameter is a
// prvalue when it participates in forwarding-reference deduction.

template<bool Condition, class T = int>
struct enable_if {};

template<class T>
struct enable_if<true, T> {
  typedef T type;
};

template<class A, class B>
struct same {
  static const bool value = false;
};

template<class A>
struct same<A, A> {
  static const bool value = true;
};

template<class F, F Value>
struct nontype_holder {};

struct target {
  int data;
  int method() { return data; }
};

template<class T>
struct forwarded_type {};

template<class R, class F>
forwarded_type<F> invoke_r(F &&);

template<class F, F Value>
struct forwarded_nontype {
  typedef decltype(invoke_r<int>(Value)) type;
};

template<class Signature>
struct function_ref;

template<class R, class A>
struct function_ref<R(A)> {
  template<class F, F Value, typename enable_if<true>::type = 0>
  function_ref(nontype_holder<F, Value>);
};

typedef decltype(&target::method) method_pointer;
typedef decltype(&target::data) data_pointer;

typedef decltype(function_ref<int(target &)>(
    nontype_holder<decltype(&target::method), &target::method>())) method_ref;
typedef decltype(function_ref<int &(target &)>(
    nontype_holder<decltype(&target::data), &target::data>())) data_ref;

static_assert(same<method_ref, function_ref<int(target &)> >::value,
              "member-function NTTP constructor deduction");
static_assert(same<data_ref, function_ref<int &(target &)> >::value,
              "data-member NTTP constructor deduction");
static_assert(same<typename forwarded_nontype<method_pointer,
                                              &target::method>::type,
                   forwarded_type<method_pointer> >::value,
              "member-function NTTP is a prvalue during forwarding deduction");
static_assert(same<typename forwarded_nontype<data_pointer,
                                              &target::data>::type,
                   forwarded_type<data_pointer> >::value,
              "data-member NTTP is a prvalue during forwarding deduction");

int main() { return 0; }
