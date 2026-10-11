case namespace-lambda-type
type namespace-lambda $_0 ns

case namespace-lambda-call
function namespace-lambda $_0 operator-call ns
qualifier const
param int

case namespace-lambda-encoding-call
function encoding
namespace-lambda-context $_0 ns
qualifier const
operator-terminal call
param int
