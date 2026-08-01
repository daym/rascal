unit u;
interface
type
  tbase = class
    procedure doit(n : longint); virtual;
  end;
  tmid = class(tbase)
  end;
  tchild = class(tmid)
    procedure doit(n : longint); reintroduce;
  end;
implementation
procedure tbase.doit(n : longint); begin end;
procedure tchild.doit(n : longint); begin end;
end.
