namespace member_template_cast_arg
{

struct buffer
{
  int value;
};

struct error_code
{
};

struct stream
{
  template<class Buffer>
  int read_some(Buffer const & buffer)
  {
    return buffer.value;
  }

  template<class Buffer>
  int read_some(Buffer const & buffer, error_code &)
  {
    return buffer.value + 1;
  }
};

struct suite
{
  template<class Condition>
  bool expect(Condition const & condition, char const *, int)
  {
    return condition != 0;
  }
};

template<class Stream>
bool check_stream(suite & tests)
{
  return tests.expect(static_cast<
      int (Stream::*)(buffer const &)>(
      &Stream::template read_some<buffer>), "", 0) &&
      tests.expect(static_cast<
      int (Stream::*)(buffer const &, error_code &)>(
      &Stream::template read_some<buffer>), "", 0);
}

}

int main()
{
  member_template_cast_arg::suite tests;
  return member_template_cast_arg::
      check_stream<member_template_cast_arg::stream>(tests) ? 0 : 1;
}
