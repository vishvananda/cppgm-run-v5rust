#include <deque>
#include <cstdlib>

extern int inline_tls_deque_constructed;
extern int inline_tls_deque_destroyed;

struct InlineTlsDequeProbe {
  std::deque<int> values;

  InlineTlsDequeProbe()
  {
    ++inline_tls_deque_constructed;
  }

  ~InlineTlsDequeProbe()
  {
    ++inline_tls_deque_destroyed;
    if(inline_tls_deque_destroyed != 1) {
      std::_Exit(80 + inline_tls_deque_destroyed);
    }
  }
};

inline int touch_inline_tls_deque_probe()
{
  static thread_local InlineTlsDequeProbe probe;
  probe.values.push_back(7);
  return static_cast<int>(probe.values.size());
}
