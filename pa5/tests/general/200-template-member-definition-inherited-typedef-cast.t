struct locale { struct facet {}; struct id {}; };
struct messages_base { typedef long catalog; };
template <class T> struct basic_string {};

template <class T>
class messages : public locale::facet, public messages_base {
public:
  static locale::id id;
protected:
  virtual catalog do_open(const basic_string<char>&, const locale&) const;
};

template <class T>
locale::id messages<T>::id;

template <class T>
typename messages<T>::catalog messages<T>::do_open(const basic_string<char>& nm, const locale&) const {
  return (catalog)0;
}
