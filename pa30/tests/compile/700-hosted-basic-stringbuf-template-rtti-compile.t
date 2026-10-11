#include <ios>
#include <sstream>
#include <streambuf>

template<class CharT, class BufferT>
class basic_pointerbuf : public BufferT {
protected:
  typedef BufferT base_type;
  typedef basic_pointerbuf<CharT, BufferT> this_type;
  typedef typename base_type::char_type char_type;
  typedef typename base_type::pos_type pos_type;
  typedef typename base_type::off_type off_type;
  typedef std::streamsize streamsize;

public:
  basic_pointerbuf() : base_type() { this_type::setbuf(0, 0); }

protected:
  inline base_type *setbuf(char_type *s, streamsize n) override;
  inline typename this_type::pos_type seekpos(pos_type sp, std::ios_base::openmode which) override;
  inline typename this_type::pos_type seekoff(off_type off, std::ios_base::seekdir way, std::ios_base::openmode which) override;
};

template<class CharT, class BufferT>
BufferT *basic_pointerbuf<CharT, BufferT>::setbuf(char_type *s, streamsize n)
{
  this->setg(s, s, s + n);
  return this;
}

template<class CharT, class BufferT>
typename basic_pointerbuf<CharT, BufferT>::pos_type
basic_pointerbuf<CharT, BufferT>::seekpos(pos_type sp, std::ios_base::openmode)
{
  return pos_type(off_type(sp));
}

template<class CharT, class BufferT>
typename basic_pointerbuf<CharT, BufferT>::pos_type
basic_pointerbuf<CharT, BufferT>::seekoff(off_type off, std::ios_base::seekdir, std::ios_base::openmode)
{
  return pos_type(off);
}

template<class BufferT, class CharT>
class basic_unlockedbuf : public basic_pointerbuf<CharT, BufferT> {
public:
  typedef basic_pointerbuf<CharT, BufferT> base_type;
  typedef typename base_type::streamsize streamsize;
  using base_type::setbuf;
};

template<class CharT, class Traits>
using stringbuffer_t = basic_unlockedbuf<std::basic_stringbuf<CharT, Traits>, CharT>;

void use_streambuf_rtti()
{
  stringbuffer_t<char, std::char_traits<char> > buffer;
  buffer.setbuf(0, 0);
}

static_assert(sizeof(&use_streambuf_rtti) > 0, "streambuf rtti anchor");
