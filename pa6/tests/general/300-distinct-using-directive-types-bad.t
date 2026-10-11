// N3485 7.3.4: declarations naming distinct types remain ambiguous.
namespace left { typedef int T; }
namespace right { using T = long; }
using namespace left;
using namespace right;
T value;
