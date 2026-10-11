struct type_info {};

struct P {
  virtual void* g(const type_info& ti [[__gnu__::__unused__]]) noexcept;
};

void* P::g(const type_info& ti [[__gnu__::__unused__]]) noexcept
{
  return 0;
}
