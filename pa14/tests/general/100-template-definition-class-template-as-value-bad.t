template<class T>
struct holder {};

template<class T>
int invalid_template_value()
{
  return holder;
}
