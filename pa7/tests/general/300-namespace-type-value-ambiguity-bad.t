// AUDIT-ID: LOOKUP-NAMESPACE-MIXED
// AUDIT-EXPECT: reject
// N3485 7.3.4/6: imported ordinary names do not hide imported class names.
namespace types { struct X; }
namespace values { int X; }
using namespace types;
using namespace values;
int main() { return X; }
