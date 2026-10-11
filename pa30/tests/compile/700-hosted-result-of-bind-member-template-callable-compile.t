#include <functional>
#include <type_traits>

struct HostedBindTemplateHandler {
  template<class... Args>
  void operator()(Args&&...) {
  }
};

template<class Target>
struct HostedAllocatorBinder {
  Target target_;

  explicit HostedAllocatorBinder(Target target) : target_(target) {}

  template<class... Args>
  typename std::result_of<Target&(Args&&...)>::type
  operator()(Args&&... args) & {
    return target_(static_cast<Args&&>(args)...);
  }

  template<class... Args>
  typename std::result_of<Target(Args&&...)>::type
  operator()(Args&&... args) && {
    return static_cast<Target&&>(target_)(static_cast<Args&&>(args)...);
  }

  template<class... Args>
  typename std::result_of<const Target&(Args&&...)>::type
  operator()(Args&&... args) const& {
    return target_(static_cast<Args&&>(args)...);
  }
};

template<class Handler>
struct HostedBinder0 {
  Handler handler_;

  explicit HostedBinder0(Handler handler) : handler_(handler) {}

  void operator()() {
    static_cast<Handler&&>(handler_)();
  }
};

int main() {
  typedef decltype(std::bind(HostedBindTemplateHandler(), 1, 2.0)) Bound;
  typedef typename std::result_of<HostedAllocatorBinder<Bound>()>::type Result;
  static_assert(std::is_same<Result, void>::value, "result_of callable binder");
  Bound bound = std::bind(HostedBindTemplateHandler(), 1, 2.0);
  HostedAllocatorBinder<Bound> wrapped(bound);
  HostedBinder0<HostedAllocatorBinder<Bound> > binder(wrapped);
  binder();
  return 0;
}
