// VALIDATION: run-pass
// N3485 focus: 14.8.2 [temp.deduct], constructor-template deduction with a
// non-deduced braced argument followed by an array-bound deduction argument.

typedef unsigned long size_t;

struct storage_ptr {
  int tag;
  storage_ptr() : tag(5) {}
};

struct parse_options {
  int max_depth;
};

struct stream_parser {
  int value;

  template<size_t N>
  stream_parser(storage_ptr sp,
                parse_options const&,
                unsigned char (&buffer)[N])
      : value((int)N + sp.tag + (buffer[0] == 0 ? 0 : 1000))
  {
  }
};

int main()
{
  parse_options opts = {1};
  unsigned char parser_buf[17] = {};
  stream_parser p({}, opts, parser_buf);
  return p.value == 22 ? 0 : 1;
}
