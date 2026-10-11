namespace outer {
namespace imported {
namespace target {
typedef int type;
}
}

namespace owner {
using namespace imported;
}

namespace alias = owner::target;
}

outer::alias::type value;
