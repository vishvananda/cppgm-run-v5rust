// Storage, linkage and declaration identity regression; frozen C++ behavior.
namespace N{extern "C" int h(int);}extern "C++" int N::h(int);
