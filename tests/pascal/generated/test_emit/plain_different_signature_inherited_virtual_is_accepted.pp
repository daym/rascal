unit u;
interface
type
  tbase = class
    procedure doit(n : longint); virtual;
  end;
  tchild = class(tbase)
    procedure doit(s : shortstring);
  end;
implementation
procedure tbase.doit(n : longint); begin end;
procedure tchild.doit(s : shortstring); begin end;
end.
