namespace base {
int f = 1;
int g = 2;
}
using namespace base;
namespace wrap { using namespace base; }

int h() { return ::f + wrap::g; }
