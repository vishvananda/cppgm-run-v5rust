// CWG 1330: member exception specifications are complete-class contexts.
struct holder
{
  void current_size() noexcept(sizeof(holder) > 0);
  void later_type() noexcept(sizeof(later) > 0);
  void permitted() throw(holder);
  void later_exception_type() throw(later);
  void later_function() noexcept(sizeof(g()) > 0);
  struct nested
  {
    void outer_size() noexcept(sizeof(holder) > 0);
  };
  typedef int later;
  static int g();
};

int main() { return 0; }
