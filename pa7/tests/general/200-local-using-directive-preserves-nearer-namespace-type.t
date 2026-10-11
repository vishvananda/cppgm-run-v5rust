namespace library {

namespace imported {
namespace detail {
typedef long unrelated;
}
}

namespace websocket {

namespace detail {
typedef int frame_header;
}

int read_header()
{
  using namespace imported;
  detail::frame_header header = 7;
  return header;
}

}
}
