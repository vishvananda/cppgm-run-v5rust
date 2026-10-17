// N3485 3.3.1p4, 14.5.4p1: a friend class template cannot replace a namespace.
// expect reject
namespace Existing {}
class Owner {
  template<class T> friend struct Existing;
};
