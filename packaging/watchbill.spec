# The package that `dnf install` takes: the commands `wb` and `watchbill`, run by
# the Python of the distribution. packaging/rpm.sh builds it from a checkout and
# passes the revision as wb_version.

Name:           watchbill
Version:        %{?wb_version}%{!?wb_version:0.0.1}
Release:        1%{?dist}
Summary:        A harness for AI agents in embedded software development

# No licence is chosen yet. This line says so, and is to be replaced when one is.
License:        LicenseRef-Not-Chosen
URL:            https://github.com/fa1k3n/watchbill
Source:         %{name}-%{version}.tar.gz

BuildArch:      noarch
BuildRequires:  python3-devel
BuildRequires:  pyproject-rpm-macros
# The tests make git repositories.
BuildRequires:  git-core
# `wb init` asks git for the root of the repository and the address of origin.
Requires:       git-core

%description
Watchbill is a harness for AI agents in embedded software development, deployed
at a fixed revision onto a project that already exists. The command is wb.

%prep
%autosetup -n %{name}-%{version}

%generate_buildrequires
%pyproject_buildrequires

%build
%pyproject_wheel

%install
%pyproject_install
%pyproject_save_files watchbill

%check
%{python3} -m unittest discover -s tests -v

%files -f %{pyproject_files}
%doc README.md
%{_bindir}/wb
%{_bindir}/watchbill
