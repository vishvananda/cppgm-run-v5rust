// A temporary that a static-storage reference binds lives as long as the
// reference (12.2p5) and is named after it (Itanium 5.1.4.5,
// `_ZGR <object name> [<seq-id>] _`). The host-built unit reads the
// course unit's references, so each temporary must outlive the course
// unit's initialization.
struct S
{
  int v;
  S(int x) : v(x) {}
  ~S() {}
};

extern const S& g;
namespace ns { extern const S& nr; }
extern const int& gi;

// One object in both units: `_ZZ6sharedvE2sr`, guarded by `_ZGVZ6sharedvE2sr`;
// each unit names the temporary `_ZGRZ6sharedvE2sr_`.
inline int shared()
{
  static const S& sr = S(14);
  return sr.v;
}
