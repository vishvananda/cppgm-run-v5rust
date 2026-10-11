// N3485 focus: 7.3.2 [namespace.alias] namespace-alias-definition
namespace N { typedef int Y; }
namespace M = N;
using namespace M;
Y x;
