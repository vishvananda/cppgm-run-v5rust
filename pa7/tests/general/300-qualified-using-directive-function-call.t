namespace base {
int f() { return 1; }
int g() { return 2; }
}
using namespace base;
namespace wrap { using namespace base; }

int h() { return ::f() + wrap::g(); }
