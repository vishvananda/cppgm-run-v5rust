// VALIDATION: compile-pass
// Host ABI compatibility: va_list parameters use their adjusted ABI type.

namespace tools {

template<class Stream>
void format_report(Stream &, __builtin_va_list)
{
}

struct logger
{
};

void report_assertion(logger & out, __builtin_va_list args)
{
  format_report(out, args);
}

}

int main()
{
  return 0;
}
