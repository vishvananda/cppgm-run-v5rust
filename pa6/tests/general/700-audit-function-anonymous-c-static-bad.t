// Frozen PA6 namespace internal-linkage declaration regression.
namespace {extern "C" {int f();static int f();}}
