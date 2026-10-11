// AUDIT-ID: LOOKUP-NAMESPACE
// AUDIT-EXPECT: compile
// N3485 7.1.3 and 7.3.4: aliases of the same type converge through
// namespace imports, including qualified, cyclic and block-scope lookup.
namespace origin { struct C {}; typedef int T; }
namespace left {
  typedef int T;
  typedef int A[2];
  typedef origin::C C;
  namespace common = origin;
}
namespace right {
  using T = int;
  using A = int[2];
  using C = origin::C;
  namespace common = origin;
}
namespace combined { using namespace left; using namespace right; }
using namespace left;
using namespace right;
T unqualified_scalar;
combined::T scalar;
combined::A array;
combined::C object;
combined::common::T shared_namespace;
namespace cycle_left { using namespace combined; }
namespace cycle_right { using namespace cycle_left; }
namespace cycle_left { using namespace cycle_right; }
cycle_left::A transitive_array;
void local() {
  using namespace left;
  using namespace right;
  T block_value;
}
