namespace first { int value; }
namespace second { int value; }
namespace both { using namespace first; using namespace second; }
using both::value __attribute__((using_if_exists));
