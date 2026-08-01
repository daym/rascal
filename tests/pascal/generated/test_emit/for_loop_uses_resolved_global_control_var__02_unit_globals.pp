unit globals;
interface
type
  ttoken = (_plus, _assignment);
var
  token : ttoken;
const
  first_overloaded = _plus;
  last_overloaded = _assignment;
implementation
end.
