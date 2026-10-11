#include <functional>
#include <type_traits>

struct HostedBindMemberSink {
  int value;
  int data;

  void run() {
    value = 7;
  }
};

template<class Function>
struct HostedBindThreadFunc {
  Function f_;

  explicit HostedBindThreadFunc(Function f) : f_(f) {}

  void run() {
    f_();
  }
};

#if defined(_LIBCPP_VERSION) && _LIBCPP_VERSION >= 210000
static_assert(std::__is_invocable_v<void (HostedBindMemberSink::*)(),
                                    HostedBindMemberSink*&>,
              "member function pointer through object pointer reference");
static_assert(std::__is_invocable_v<void (HostedBindMemberSink::* const)(),
                                    HostedBindMemberSink*&>,
              "top-level cv member function pointer");
static_assert(std::__is_invocable_r_v<int&,
                                      int HostedBindMemberSink::*,
                                      HostedBindMemberSink*>,
              "data member pointer result");
static_assert(std::__is_invocable_r_v<const int&,
                                      int HostedBindMemberSink::*,
                                      const HostedBindMemberSink*>,
              "const data member pointer result");

void hosted_bind_member_pointer_invocable() {
  HostedBindMemberSink sink = {0, 3};
  std::__1::__bind<void (HostedBindMemberSink::*)(), HostedBindMemberSink*> bound(
      &HostedBindMemberSink::run,
      &sink);
  HostedBindThreadFunc<
      std::__1::__bind<void (HostedBindMemberSink::*)(), HostedBindMemberSink*> > func(bound);
  func.run();
}
#endif

int main() {
#if defined(_LIBCPP_VERSION) && _LIBCPP_VERSION >= 210000
  hosted_bind_member_pointer_invocable();
#endif
  return 0;
}
