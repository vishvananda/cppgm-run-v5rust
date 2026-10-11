// HHC-093
struct exception {
  virtual ~exception();
  virtual const char* what() const;
};

struct bad_alloc : exception {
  ~bad_alloc() override;
  const char* what() const override;
};

struct bad_array_new_length : bad_alloc {
  ~bad_array_new_length() override;
  const char* what() const override;
};

int main() { return 0; }
