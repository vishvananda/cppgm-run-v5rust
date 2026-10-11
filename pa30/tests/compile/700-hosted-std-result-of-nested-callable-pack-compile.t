#include <type_traits>

struct HostedResultOfFiber {
  HostedResultOfFiber() {}
  HostedResultOfFiber(HostedResultOfFiber&&) {}
};

struct HostedResultOfThreadBase {};

namespace hosted_result_of_detail {

template<class Fn, class... Args>
typename std::enable_if<
    std::is_member_pointer<typename std::decay<Fn>::type>::value,
    typename std::result_of<Fn&&(Args&&...)>::type
>::type invoke(Fn&& fn, Args&&... args) {
  return fn(static_cast<Args&&>(args)...);
}

template<class Fn, class... Args>
typename std::enable_if<
    !std::is_member_pointer<typename std::decay<Fn>::type>::value,
    typename std::result_of<Fn&&(Args&&...)>::type
>::type invoke(Fn&& fn, Args&&... args) {
  return static_cast<Fn&&>(fn)(static_cast<Args&&>(args)...);
}

}  // namespace hosted_result_of_detail

template<class Executor, class Function, class Handler>
struct HostedResultOfSpawnEntryPoint {
  Function function;
  Handler handler;

  void operator()(HostedResultOfThreadBase* thread) {
    function(thread);
    handler(0);
  }
};

struct HostedResultOfFiberThread : HostedResultOfThreadBase {
  typedef HostedResultOfFiber fiber_type;

  template<class Function>
  class entry_point {
  public:
    entry_point(Function f, HostedResultOfFiberThread** out)
        : function_(f), out_(out) {}

    fiber_type operator()(fiber_type&& caller) {
      Function function(static_cast<Function&&>(function_));
      HostedResultOfFiberThread spawned;
      *out_ = &spawned;
      function(&spawned);
      return static_cast<fiber_type&&>(caller);
    }

  private:
    Function function_;
    HostedResultOfFiberThread** out_;
  };

  template<class F>
  static void spawn(F&& f) {
    HostedResultOfFiberThread* thread = 0;
    entry_point<typename std::decay<F>::type> fn(static_cast<F&&>(f), &thread);
    fiber_type caller;
    fiber_type out =
        hosted_result_of_detail::invoke(fn, static_cast<fiber_type&&>(caller));
    (void)out;
  }
};

void hosted_result_of_body(HostedResultOfThreadBase*) {}

struct HostedResultOfHandler {
  int* seen;

  void operator()(int) {
    *seen = 1;
  }
};

int main() {
  int seen = 0;
  HostedResultOfSpawnEntryPoint<int,
                                void (*)(HostedResultOfThreadBase*),
                                HostedResultOfHandler> f = {
      hosted_result_of_body,
      {&seen}};
  HostedResultOfFiberThread::spawn(f);
  return seen == 1 ? 0 : 0;
}
