unit u;
interface
type
  tpath = class
    procedure addpath(s : string; addfirst : boolean); overload;
    procedure addpath(srcpath, s : string; addfirst : boolean); overload;
  end;
procedure demo(p : tpath; s : string);
implementation
procedure tpath.addpath(s : string; addfirst : boolean);
begin
end;
procedure tpath.addpath(srcpath, s : string; addfirst : boolean);
begin
end;
procedure demo(p : tpath; s : string);
begin
  p.addpath(s, false);
end;
end.
