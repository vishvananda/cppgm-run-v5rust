// Storage, linkage and declaration identity regression; frozen C++ behavior.
namespace N{extern "C" int x;}extern "C++" int N::x;
