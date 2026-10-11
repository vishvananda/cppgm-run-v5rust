#ifdef __clang__
const char* compiler_name()
{
  return "Clang version " __clang_version__;
}
#else
const char* compiler_name()
{
  return "GNU C++ version " __VERSION__;
}
#endif

const char* selected_compiler_name = compiler_name();
