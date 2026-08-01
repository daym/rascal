unit base;
interface
type
  thandler = class
  protected
    type
      tstate = record
        value : integer;
      end;
    class procedure bar(var s : tstate); virtual;
  end;
implementation
class procedure thandler.bar(var s : tstate);
begin
  s.value := 1;
end;
end.
