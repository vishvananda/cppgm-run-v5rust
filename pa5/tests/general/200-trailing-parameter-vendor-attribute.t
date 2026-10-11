struct type_info {};

struct P {
  virtual void* g(const type_info& ti [[vendor::unused]]) noexcept;
};
