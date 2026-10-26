// PA27 explicit object spelling collides with a demanded RTTI declaration;
// expect compile reject. A pending ordinary source declaration may still be defined.
namespace std { class type_info {}; }
const std::type_info& information() { return typeid(int); }
extern "C" const char replacement[16] __asm__("_ZTIi") = {};
