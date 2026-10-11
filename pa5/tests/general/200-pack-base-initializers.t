template<class... CMixins>
class CX : public CMixins... {
public:
  CX(const CMixins&... mixins) : CMixins(mixins)... {}
  virtual ~CX();
};
