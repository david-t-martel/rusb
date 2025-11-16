function build_serial_mex(varargin)
% Simplified input parsing
p = inputParser;
addParameter(p, 'libusb_include', '', @ischar);
addParameter(p, 'libusb_lib', '', @ischar);
addParameter(p, 'debug', false, @islogical);
parse(p, varargin{:});

% Core paths setup
proj_root = fileparts(pwd);
builddir = fullfile(pwd, 'build');
outdir = fullfile(pwd, 'release');

% Ensure clean build environment
if exist(builddir, 'dir'), rmdir(builddir, 's'); end
if exist(outdir, 'dir'), rmdir(outdir, 's'); end
mkdir(builddir);
mkdir(outdir);

% Platform-specific settings
if ispc
	[includedir, libdir, libname] = get_windows_paths(p.Results, proj_root);
	compflags = {'/EHsc', '/std:c++17', '/W4'};
	if p.Results.debug
		compflags = [compflags, {'/Z7', '/Od', '/MDd'}];
	else
		compflags = [compflags, {'/O2', '/MD'}];
	end
else
	[includedir, libdir, libname] = get_unix_paths(p.Results, proj_root);
	compflags = {'-std=c++17', '-Wall', '-Wextra'};
	if p.Results.debug
		compflags = [compflags, {'-g', '-O0'}];
	else
		compflags = [compflags, {'-O2'}];
	end
end

% Construct MEX command
cmd = {
	% Source files
	fullfile(pwd, 'serial_mex.cpp')
	fullfile(pwd, 'serial_comm.c')

	% Compiler flags
	['-DMATLAB_MEX_FILE']
	['-I' includedir]
	['-I' fullfile(matlabroot, 'extern', 'include')]
	['-L' libdir]
	['-l' libname]

	% Output directory
	['-outdir' outdir]

	% Debug/Release flags
	['COMPFLAGS=$COMPFLAGS ' strjoin(compflags, ' ')]

	% Additional platform-specific flags
	['CXXFLAGS=$CXXFLAGS ' strjoin(compflags, ' ')]
	};

% Build
try
	mex(cmd{:});
	verify_build(outdir);
catch ME
	cleanup_build([builddir outdir]);
	rethrow(ME);
end
end

function [includedir, libdir, libname] = get_windows_paths(params, root)
if ~isempty(params.libusb_include)
	includedir = params.libusb_include;
else
	includedir = locate_path({
		fullfile(root, 'include'),
		'C:\libusb\include'
		});
end

if ~isempty(params.libusb_lib)
	libdir = params.libusb_lib;
else
	libdir = locate_path({
		fullfile(root, 'MS64'),
		'C:\libusb\lib'
		});
end

libname = 'libusb-1.0';

% Verify required files exist
verify_files({
	fullfile(includedir, 'libusb-1.0', 'libusb.h'),
	fullfile(libdir, 'libusb-1.0.lib'),
	fullfile(libdir, 'libusb-1.0.dll')
	});
end

function path = locate_path(candidates)
for p = candidates
	if exist(p{1}, 'dir')
		path = p{1};
		return
	end
end
error('Could not locate path in candidates: %s', strjoin(candidates, ', '));
end

function verify_files(files)
missing = files(~cellfun(@(f) exist(f, 'file'), files));
if ~isempty(missing)
	error('Missing required files:\n%s', strjoin(missing, '\n'));
end
end

function verify_build(outdir)
mexfile = fullfile(outdir, ['serial_mex.' mexext]);
assert(exist(mexfile, 'file') == 3, 'MEX file not created');
end

function cleanup_build(dirs)
cellfun(@(d) rmdir(d, 's'), dirs(cellfun(@(d) exist(d, 'dir'), dirs)));
end

function [include_path, lib_path] = get_unix_paths(params, curr_dir, libusb_root)
if isempty(params.libusb_include)
	include_candidates = {
		fullfile(curr_dir, 'include'),
		fullfile(libusb_root, 'include'),
		'/usr/include',
		'/usr/local/include'
		};
	include_path = find_valid_path(include_candidates);
else
	include_path = validate_path(params.libusb_include);
end

if isempty(params.libusb_lib)
	lib_candidates = {
		fullfile(curr_dir, 'lib'),
		fullfile(libusb_root, 'lib'),
		'/usr/lib',
		'/usr/local/lib'
		};
	lib_path = find_valid_path(lib_candidates);
else
	lib_path = validate_path(params.libusb_lib);
end
end

function path = find_valid_path(candidates)
for i = 1:length(candidates)
	if exist(candidates{i}, 'dir')
		path = candidates{i};
		return;
	end
end
error('No valid path found among candidates: %s', strjoin(candidates, ', '));
end

function path = validate_path(path)
if ~exist(path, 'dir')
	error('Directory not found: %s', path);
end
end